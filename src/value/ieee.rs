// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-148 `quire.value.ieee754-2019-default/v1`: binary32/binary64 bit-pattern
//! values, the five IEEE rounding directions plus strict `exact`, deterministic
//! NaN propagation, operation-local exception flags, the three distinct
//! comparison intrinsics, explicit conversions, package admission and I13
//! negotiation.
//!
//! Every value is an exact bit pattern. Arithmetic decodes finite operands into
//! exact dyadic integers, forms the exact real intermediate with arbitrary
//! precision integers (a truncated integer square root plus a sticky bit for
//! `sqrt`) and rounds exactly once. No host floating-point value or operation
//! exists anywhere on this path.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer as _;
use num_traits::{One, Zero};

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::comparison::{IllTyped, IllTypedCause};
use super::decimal::{Decimal, DecimalType, RoundingMode};
use super::definition::{
    CatalogRole, DefinitionLock, DefinitionReference, PackageCause, PackageRefusal,
    PackageRefusalCode, DIGEST_DOMAIN,
};
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::{Rational, RationalDomain};
use quire_exact::{Integer, IntegerInterval};

/// The IEEE profile definition identity.
pub const IEEE_DEFINITION: &str = "quire.value.ieee754-2019-default/v1";

/// An IEEE 754-2019 binary interchange width.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IeeeWidth {
    /// `binary32` (`float32`).
    Binary32,
    /// `binary64` (`float64`).
    Binary64,
}

impl IeeeWidth {
    /// Both widths.
    pub const ALL: [Self; 2] = [Self::Binary32, Self::Binary64];

    /// Total bit width.
    pub fn bits(self) -> u32 {
        match self {
            Self::Binary32 => 32,
            Self::Binary64 => 64,
        }
    }

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Binary32 => "binary32",
            Self::Binary64 => "binary64",
        }
    }

    fn format(self) -> Format {
        match self {
            Self::Binary32 => Format {
                exponent_bits: 8,
                fraction_bits: 23,
            },
            Self::Binary64 => Format {
                exponent_bits: 11,
                fraction_bits: 52,
            },
        }
    }
}

/// An IEEE value: an exact bit pattern of one width. Every pattern, including
/// every NaN sign and payload, is representable.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IeeeValue {
    width: IeeeWidth,
    bits: u64,
}

impl IeeeValue {
    /// A binary32 value from its exact bits.
    pub fn binary32(bits: u32) -> Self {
        Self {
            width: IeeeWidth::Binary32,
            bits: u64::from(bits),
        }
    }

    /// A binary64 value from its exact bits.
    pub fn binary64(bits: u64) -> Self {
        Self {
            width: IeeeWidth::Binary64,
            bits,
        }
    }

    /// The width.
    pub fn width(self) -> IeeeWidth {
        self.width
    }

    /// The exact bits, zero-extended to `u64` for binary32.
    pub fn bits(self) -> u64 {
        self.bits
    }

    /// Whether the value is any NaN.
    pub fn is_nan(self) -> bool {
        matches!(decode(self), Class::Nan { .. })
    }

    /// Whether the value is a signaling NaN.
    pub fn is_signaling_nan(self) -> bool {
        matches!(
            decode(self),
            Class::Nan {
                signaling: true,
                ..
            }
        )
    }

    /// The `totalOrder` key: `not(b)` when the sign bit is one, otherwise
    /// `b xor sign_mask`, within the value's width.
    pub fn total_order_key(self) -> u64 {
        let format = self.width.format();
        if self.bits & format.sign_mask() != 0 {
            !self.bits & format.all_mask()
        } else {
            self.bits ^ format.sign_mask()
        }
    }
}

/// One member of the closed IEEE exception flag vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IeeeFlag {
    /// `invalid`.
    Invalid,
    /// `divide_by_zero`.
    DivideByZero,
    /// `overflow`.
    Overflow,
    /// `underflow`.
    Underflow,
    /// `inexact`.
    Inexact,
}

impl IeeeFlag {
    /// Every flag in vocabulary order.
    pub const ALL: [Self; 5] = [
        Self::Invalid,
        Self::DivideByZero,
        Self::Overflow,
        Self::Underflow,
        Self::Inexact,
    ];

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::DivideByZero => "divide_by_zero",
            Self::Overflow => "overflow",
            Self::Underflow => "underflow",
            Self::Inexact => "inexact",
        }
    }

    fn mask(self) -> u8 {
        match self {
            Self::Invalid => 1,
            Self::DivideByZero => 2,
            Self::Overflow => 4,
            Self::Underflow => 8,
            Self::Inexact => 16,
        }
    }
}

/// A fresh, operation-local flag set. There is no sticky global state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct IeeeFlags(u8);

impl IeeeFlags {
    /// The empty set.
    pub const EMPTY: Self = Self(0);

    /// Whether `flag` is raised.
    pub fn contains(self, flag: IeeeFlag) -> bool {
        self.0 & flag.mask() != 0
    }

    /// Whether no flag is raised.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The raised flags in vocabulary order.
    pub fn iter(self) -> impl Iterator<Item = IeeeFlag> {
        IeeeFlag::ALL
            .into_iter()
            .filter(move |flag| self.contains(*flag))
    }

    fn with(self, flag: IeeeFlag) -> Self {
        Self(self.0 | flag.mask())
    }
}

impl FromIterator<IeeeFlag> for IeeeFlags {
    fn from_iter<I: IntoIterator<Item = IeeeFlag>>(flags: I) -> Self {
        flags.into_iter().fold(Self::EMPTY, Self::with)
    }
}

/// An exact integer, rational or decimal scalar where IEEE values meet exact
/// values: an explicit conversion source, or an operand type checking refuses.
#[derive(Clone, Copy, Debug)]
pub enum ExactScalar<'a> {
    /// An exact integer.
    Integer(&'a Integer),
    /// An exact rational.
    Rational(&'a Rational),
    /// An exact decimal with its retained representation.
    Decimal(&'a Decimal),
}

impl<'a> From<&'a Integer> for ExactScalar<'a> {
    fn from(value: &'a Integer) -> Self {
        Self::Integer(value)
    }
}

impl<'a> From<&'a Rational> for ExactScalar<'a> {
    fn from(value: &'a Rational) -> Self {
        Self::Rational(value)
    }
}

impl<'a> From<&'a Decimal> for ExactScalar<'a> {
    fn from(value: &'a Decimal) -> Self {
        Self::Decimal(value)
    }
}

impl ExactScalar<'_> {
    /// `ieee.operands` size of the source: `bits(n)`, `maxparts`, or
    /// `max(bits(coefficient), bits(10^scale))` of the retained decimal.
    fn operand_bits(self) -> u64 {
        match self {
            Self::Integer(value) => value.magnitude_bits(),
            Self::Rational(value) => value.max_part_bits(),
            Self::Decimal(value) => {
                let retained = value.representation();
                // `bits(10^scale)` with `scale <= u32::MAX` is below `2^35`.
                let power_bits = Integer::power_product_bits(
                    &Integer::one(),
                    &Integer::from(10_i64),
                    &Integer::from(u64::from(retained.scale())),
                )
                .to_u64()
                .expect("bits(10^scale) of a u32 scale fits u64");
                retained.coefficient().magnitude_bits().max(power_bits)
            }
        }
    }

    fn exact_value(self) -> Rational {
        match self {
            Self::Integer(value) => Rational::from_integer(value.clone()),
            Self::Rational(value) => value.clone(),
            Self::Decimal(value) => value.representation().to_rational(),
        }
    }
}

/// An operand as written at an IEEE operation or comparison. Only
/// [`IeeeOperand::Ieee`] operands of one width type-check.
#[derive(Clone, Copy, Debug)]
pub enum IeeeOperand<'a> {
    /// An IEEE value.
    Ieee(IeeeValue),
    /// An exact value; mixing it with IEEE operands is `ill_typed`.
    Exact(ExactScalar<'a>),
}

impl From<IeeeValue> for IeeeOperand<'_> {
    fn from(value: IeeeValue) -> Self {
        Self::Ieee(value)
    }
}

impl<'a> From<ExactScalar<'a>> for IeeeOperand<'a> {
    fn from(value: ExactScalar<'a>) -> Self {
        Self::Exact(value)
    }
}

/// An arithmetic operation. Operands default to [`IeeeValue`]; any operand
/// convertible to [`IeeeOperand`] is accepted and type-checked.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IeeeOperation<O = IeeeValue> {
    /// `a + b`.
    Add(O, O),
    /// `a - b`.
    Subtract(O, O),
    /// `a * b`.
    Multiply(O, O),
    /// `a / b`.
    Divide(O, O),
    /// `quire::value::ieee::sqrt(a)`.
    SquareRoot(O),
    /// `quire::value::ieee::fma(a, b, c)`: `a * b + c` with one rounding.
    FusedMultiplyAdd(O, O, O),
}

impl<O> IeeeOperation<O> {
    /// The operation's kind.
    pub fn kind(&self) -> IeeeOperationKind {
        match self {
            Self::Add(..) => IeeeOperationKind::Add,
            Self::Subtract(..) => IeeeOperationKind::Subtract,
            Self::Multiply(..) => IeeeOperationKind::Multiply,
            Self::Divide(..) => IeeeOperationKind::Divide,
            Self::SquareRoot(..) => IeeeOperationKind::SquareRoot,
            Self::FusedMultiplyAdd(..) => IeeeOperationKind::FusedMultiplyAdd,
        }
    }

    fn try_map<P, E>(self, mut f: impl FnMut(O) -> Result<P, E>) -> Result<IeeeOperation<P>, E> {
        Ok(match self {
            Self::Add(a, b) => IeeeOperation::Add(f(a)?, f(b)?),
            Self::Subtract(a, b) => IeeeOperation::Subtract(f(a)?, f(b)?),
            Self::Multiply(a, b) => IeeeOperation::Multiply(f(a)?, f(b)?),
            Self::Divide(a, b) => IeeeOperation::Divide(f(a)?, f(b)?),
            Self::SquareRoot(a) => IeeeOperation::SquareRoot(f(a)?),
            Self::FusedMultiplyAdd(a, b, c) => IeeeOperation::FusedMultiplyAdd(f(a)?, f(b)?, f(c)?),
        })
    }
}

/// The IEEE value of a written operand; an exact operand is `ill_typed`.
fn ieee_operand<'a>(operand: impl Into<IeeeOperand<'a>>) -> Result<IeeeValue, IllTyped> {
    match operand.into() {
        IeeeOperand::Ieee(value) => Ok(value),
        IeeeOperand::Exact(_) => Err(IllTyped {
            cause: IllTypedCause::IeeeWithExactOperand,
        }),
    }
}

impl IeeeOperation {
    fn arity(self) -> u64 {
        match self {
            Self::Add(..) | Self::Subtract(..) | Self::Multiply(..) | Self::Divide(..) => 2,
            Self::SquareRoot(..) => 1,
            Self::FusedMultiplyAdd(..) => 3,
        }
    }

    /// The first operand and the rest, for the width check.
    fn operands(self) -> (IeeeValue, Vec<IeeeValue>) {
        match self {
            Self::Add(a, b) | Self::Subtract(a, b) | Self::Multiply(a, b) | Self::Divide(a, b) => {
                (a, vec![b])
            }
            Self::SquareRoot(a) => (a, Vec::new()),
            Self::FusedMultiplyAdd(a, b, c) => (a, vec![b, c]),
        }
    }
}

/// A comparison intrinsic. The three are distinct selected operations.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IeeeComparison {
    /// `quire::value::ieee::numericEqual`.
    NumericEqual,
    /// `quire::value::ieee::totalOrder(a, b)`: `a` precedes or equals `b`.
    TotalOrder,
    /// `quire::value::ieee::bitIdentical`.
    BitIdentical,
}

impl IeeeComparison {
    /// Every comparison.
    pub const ALL: [Self; 3] = [Self::NumericEqual, Self::TotalOrder, Self::BitIdentical];

    /// The operation's kind.
    pub fn kind(self) -> IeeeOperationKind {
        match self {
            Self::NumericEqual => IeeeOperationKind::NumericEqual,
            Self::TotalOrder => IeeeOperationKind::TotalOrder,
            Self::BitIdentical => IeeeOperationKind::BitIdentical,
        }
    }
}

/// Every IEEE operation a package item may require of a backend.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IeeeOperationKind {
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Square root.
    SquareRoot,
    /// Fused multiply-add.
    FusedMultiplyAdd,
    /// Numeric equality.
    NumericEqual,
    /// IEEE `totalOrder`.
    TotalOrder,
    /// Bit identity.
    BitIdentical,
    /// Explicit cross-width conversion.
    ConvertWidth,
    /// Explicit conversion to an exact rational.
    ToExact,
    /// Explicit conversion from an exact rational.
    FromExact,
}

impl IeeeOperationKind {
    /// Every kind.
    pub const ALL: [Self; 12] = [
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::SquareRoot,
        Self::FusedMultiplyAdd,
        Self::NumericEqual,
        Self::TotalOrder,
        Self::BitIdentical,
        Self::ConvertWidth,
        Self::ToExact,
        Self::FromExact,
    ];

    /// The reserved qualified intrinsic identity, for the five FR-148 intrinsics.
    pub fn intrinsic_identity(self) -> Option<&'static str> {
        match self {
            Self::SquareRoot => Some("quire::value::ieee::sqrt"),
            Self::FusedMultiplyAdd => Some("quire::value::ieee::fma"),
            Self::NumericEqual => Some("quire::value::ieee::numericEqual"),
            Self::TotalOrder => Some("quire::value::ieee::totalOrder"),
            Self::BitIdentical => Some("quire::value::ieee::bitIdentical"),
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::ConvertWidth
            | Self::ToExact
            | Self::FromExact => None,
        }
    }

    /// Whether the operation applies a rounding direction.
    pub fn rounds(self) -> bool {
        match self {
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::SquareRoot
            | Self::FusedMultiplyAdd
            | Self::ConvertWidth
            | Self::FromExact => true,
            Self::NumericEqual | Self::TotalOrder | Self::BitIdentical | Self::ToExact => false,
        }
    }
}

/// Every reserved qualified intrinsic identity.
pub fn ieee_intrinsic_identities() -> impl Iterator<Item = &'static str> {
    IeeeOperationKind::ALL
        .into_iter()
        .filter_map(IeeeOperationKind::intrinsic_identity)
}

/// The run provenance of one IEEE result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IeeeProvenance {
    width: IeeeWidth,
    rounding: RoundingMode,
}

impl IeeeProvenance {
    /// The result width.
    pub fn width(self) -> IeeeWidth {
        self.width
    }

    /// The selected rounding direction or strict `exact` policy.
    pub fn rounding(self) -> RoundingMode {
        self.rounding
    }

    /// The selected IEEE definition identity.
    pub fn definition(self) -> &'static str {
        IEEE_DEFINITION
    }
}

/// A completed IEEE bit pattern with its fresh flag set.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IeeeResult {
    value: IeeeValue,
    flags: IeeeFlags,
    provenance: IeeeProvenance,
}

impl IeeeResult {
    /// The result bits.
    pub fn value(&self) -> IeeeValue {
        self.value
    }

    /// The operation-local flags.
    pub fn flags(&self) -> IeeeFlags {
        self.flags
    }

    /// Width, rounding policy and definition.
    pub fn provenance(&self) -> IeeeProvenance {
        self.provenance
    }
}

/// Information an IEEE-to-exact conversion discards.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IeeeExactLoss {
    /// The source was `-0`, whose sign has no rational representation.
    NegativeZeroSign,
}

/// The exact value of a finite IEEE operand.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IeeeExact {
    value: Rational,
    loss: Option<IeeeExactLoss>,
}

impl IeeeExact {
    /// The exact rational.
    pub fn value(&self) -> &Rational {
        &self.value
    }

    /// The discarded information, reported as loss.
    pub fn loss(&self) -> Option<IeeeExactLoss> {
        self.loss
    }
}

/// An IEEE package whose profile definition closure was admitted. Only
/// [`DefinitionLock::admit_ieee_profile`] constructs it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AdmittedIeeeProfile {
    definition: DefinitionReference,
}

impl AdmittedIeeeProfile {
    /// The retained, admitted DefinitionRef.
    pub fn definition(&self) -> &DefinitionReference {
        &self.definition
    }
}

impl DefinitionLock {
    /// Admit a checked package's IEEE profile closure.
    ///
    /// `retained` lists every DefinitionRef the package retains as IEEE policy;
    /// `declarations` lists the qualified identities of user declarations. A
    /// missing, repeated or mismatched profile, or a user declaration bound to a
    /// reserved intrinsic identity, is `refused { code: invalid_package }`.
    pub fn admit_ieee_profile(
        &self,
        retained: &[DefinitionReference],
        declarations: &[&str],
    ) -> Result<AdmittedIeeeProfile, PackageRefusal> {
        let expected = self
            .entry(CatalogRole::IeeeProfile)
            .ok_or(invalid_package(PackageCause::MissingMember))?;
        for reference in retained {
            let cause = if reference.identity != expected.identity
                || reference.authority != expected.authority
            {
                Some(PackageCause::IncompatibleDefinition)
            } else if reference.revision.namespace != expected.revision_namespace
                || reference.revision.value != expected.revision_value
            {
                Some(PackageCause::RevisionMismatch)
            } else if reference.digest_domain != DIGEST_DOMAIN {
                Some(PackageCause::DigestDomainMismatch)
            } else {
                None
            };
            if let Some(cause) = cause {
                return Err(invalid_package(cause));
            }
        }
        let definition = match retained {
            [] => return Err(invalid_package(PackageCause::MissingMember)),
            [definition] => definition.clone(),
            [_, _, ..] => return Err(invalid_package(PackageCause::ConflictingDefinition)),
        };
        // FR-148: a user declaration bound to a reserved intrinsic identity is
        // `invalid_package` with cause `conflicting-definition`.
        if declarations
            .iter()
            .any(|declared| ieee_intrinsic_identities().any(|reserved| reserved == *declared))
        {
            return Err(invalid_package(PackageCause::ConflictingDefinition));
        }
        Ok(AdmittedIeeeProfile { definition })
    }
}

fn invalid_package(cause: PackageCause) -> PackageRefusal {
    PackageRefusal {
        code: PackageRefusalCode::InvalidPackage,
        cause,
    }
}

/// Evaluate one arithmetic operation under `rounding`.
///
/// Operands of different widths, or an exact operand, are ill-typed and
/// consume nothing. Otherwise the result is completed bits and flags, a
/// strict-`exact` refusal carrying only the would-be flags, or incomplete with
/// no bits or flags.
pub fn evaluate_ieee<'a, O: Into<IeeeOperand<'a>>>(
    _profile: &AdmittedIeeeProfile,
    operation: IeeeOperation<O>,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Result<Outcome<IeeeResult>, IllTyped> {
    let operation = operation.try_map(ieee_operand)?;
    let (first, rest) = operation.operands();
    let width = same_width(first, &rest)?;
    Ok(Outcome::from_stop(arithmetic(
        operation, width, rounding, meter,
    )))
}

/// Evaluate one comparison intrinsic. Cross-width or exact operands are
/// ill-typed and consume nothing.
pub fn compare_ieee<'a>(
    _profile: &AdmittedIeeeProfile,
    comparison: IeeeComparison,
    left: impl Into<IeeeOperand<'a>>,
    right: impl Into<IeeeOperand<'a>>,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    let (left, right) = (ieee_operand(left)?, ieee_operand(right)?);
    // FR-148: every comparison whose IEEE operands differ in width is
    // `ill_typed` before any charge.
    let width = same_width(left, &[right])?;
    Ok(Outcome::from_stop(compare(
        comparison, left, right, width, meter,
    )))
}

fn compare(
    comparison: IeeeComparison,
    left: IeeeValue,
    right: IeeeValue,
    width: IeeeWidth,
    meter: &mut Meter,
) -> Result<bool, Stop> {
    charge_operands(meter, width, 2)?;
    let result = match comparison {
        IeeeComparison::NumericEqual => match (decode(left), decode(right)) {
            (Class::Nan { .. }, _) | (_, Class::Nan { .. }) => false,
            (Class::Zero { .. }, Class::Zero { .. }) => true,
            (Class::Infinite { .. } | Class::Zero { .. } | Class::Finite(_), _) => {
                left.bits == right.bits
            }
        },
        IeeeComparison::TotalOrder => left.total_order_key() <= right.total_order_key(),
        IeeeComparison::BitIdentical => left.bits == right.bits,
    };
    charge_result(meter)?;
    Ok(result)
}

/// Explicitly convert `value` to `target` width under `rounding`.
///
/// Finite values round once with flags; a NaN keeps its sign and integer
/// payload, is quieted, and raises `invalid` when signaling. A payload the
/// target cannot hold is refused rather than truncated.
pub fn convert_ieee_width(
    _profile: &AdmittedIeeeProfile,
    value: IeeeValue,
    target: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Outcome<IeeeResult> {
    Outcome::from_stop(convert_width(value, target, rounding, meter))
}

/// The type an explicit IEEE-to-exact conversion names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IeeeExactTarget<'a> {
    /// A grammar-named `Rational[lo, hi; dmin, dmax]`, the only defined target.
    Rational(&'a RationalDomain),
    /// A `Decimal[..]` type; a direct conversion is ill-typed.
    Decimal(&'a DecimalType),
    /// The unbounded `Integer`; a direct conversion is ill-typed.
    Integer,
    /// A bounded `Int[..]`; a direct conversion is ill-typed.
    BoundedInteger(&'a IntegerInterval),
}

/// Explicitly convert a finite IEEE value to a `Rational[..]` target. NaN and
/// the infinities have no exact value and are undefined; an exact value
/// outside the target domain is refused before it is retained. A direct
/// `Decimal`, `Integer` or `Int[..]` target is ill-typed with no charge.
pub fn ieee_to_exact(
    _profile: &AdmittedIeeeProfile,
    value: IeeeValue,
    target: IeeeExactTarget<'_>,
    meter: &mut Meter,
) -> Result<Outcome<IeeeExact>, IllTyped> {
    let domain = match target {
        IeeeExactTarget::Rational(domain) => domain,
        IeeeExactTarget::Decimal(_)
        | IeeeExactTarget::Integer
        | IeeeExactTarget::BoundedInteger(_) => {
            return Err(IllTyped {
                cause: IllTypedCause::IeeeToNonRationalExact,
            })
        }
    };
    Ok(Outcome::from_stop(to_exact(value, domain, meter)))
}

fn to_exact(
    value: IeeeValue,
    domain: &RationalDomain,
    meter: &mut Meter,
) -> Result<IeeeExact, Stop> {
    // `value-accounting.md`: `ieee.operands` at the source width, then for a
    // finite value `ieee.exact-intermediate` at the result's `maxparts`, then
    // uncharged `Rational[..]` membership, then `ieee.result-retain`.
    charge_operands(meter, value.width, 1)?;
    let exact = match decode(value) {
        Class::Nan { .. } | Class::Infinite { .. } => {
            return Err(Stop::Undefined(Undefined::IeeeNotFinite))
        }
        Class::Zero { negative } => {
            // `maxparts(0/1) = 1`.
            charge_exact_result(meter, 1)?;
            IeeeExact {
                value: Rational::from_integer(Integer::zero()),
                loss: negative.then_some(IeeeExactLoss::NegativeZeroSign),
            }
        }
        Class::Finite(finite) => {
            charge_exact_result(meter, finite.max_part_bits())?;
            IeeeExact {
                value: finite.to_rational(),
                loss: None,
            }
        }
    };
    // Membership charges nothing and refuses before the result is retained.
    if !domain.contains(&exact.value) {
        return Err(Stop::Refused(Refusal::IeeeRationalOutOfDomain));
    }
    charge_result(meter)?;
    Ok(exact)
}

/// Explicitly convert an exact integer, rational or decimal to `width` under
/// `rounding`, reporting loss through the IEEE flags; strict `exact` refuses
/// any loss.
pub fn exact_to_ieee<'a>(
    _profile: &AdmittedIeeeProfile,
    source: impl Into<ExactScalar<'a>>,
    width: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Outcome<IeeeResult> {
    Outcome::from_stop(from_exact(source.into(), width, rounding, meter))
}

fn from_exact(
    source: ExactScalar<'_>,
    width: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Result<IeeeResult, Stop> {
    // The source is measured on its own kind, a decimal on its retained
    // representation; the intermediate and the rounding are charged at the
    // target width.
    charge_operand_bits(meter, source.operand_bits(), 1)?;
    charge_width(meter, ChargePoint::IeeeExactIntermediate, width)?;
    let value = &source.exact_value();
    let negative = value.numerator().is_negative();
    let magnitude = value.numerator().as_big().magnitude().clone();
    let exact = if magnitude.is_zero() {
        Exact::Zero { negative: false }
    } else {
        let denominator = value.denominator().as_big().magnitude().clone();
        Exact::Real {
            negative,
            approximation: approximate(&magnitude, &denominator, 0, width.format()),
        }
    };
    finish_rounding(meter, exact, width, rounding)
}

/// A backend's IEEE capabilities offered during I13 negotiation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IeeeBackendCapabilities {
    /// Supported widths.
    pub widths: BTreeSet<IeeeWidth>,
    /// Supported operations.
    pub operations: BTreeSet<IeeeOperationKind>,
    /// Supported rounding directions, including strict `exact` when offered.
    pub roundings: BTreeSet<RoundingMode>,
    /// Whether the backend implements this profile's NaN and flag policy.
    pub exceptional_policy: bool,
    /// Whether the backend can discharge a required finite resource proof.
    pub finite_proof: bool,
}

/// One package item's IEEE requirement.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IeeeItemRequirement {
    /// Selected width.
    pub width: IeeeWidth,
    /// Required operation.
    pub operation: IeeeOperationKind,
    /// Selected rounding policy (ignored by non-rounding operations).
    pub rounding: RoundingMode,
    /// Whether finite execution needs a resource proof for this item.
    pub requires_finite_proof: bool,
}

/// What the backend lacks for an `unsupported` item.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IeeeUnsupportedCause {
    /// The selected width.
    Width(IeeeWidth),
    /// The required operation or intrinsic.
    Operation(IeeeOperationKind),
    /// The selected rounding direction or strict policy.
    Rounding(RoundingMode),
    /// The NaN/flag policy.
    ExceptionalPolicy,
}

/// The per-item I13 negotiation disposition. It is not an evaluator outcome
/// and never changes package admission or selects a substitute evaluator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IeeeDisposition {
    /// The backend implements the item exactly.
    Supported,
    /// `unsupported`.
    Unsupported(IeeeUnsupportedCause),
    /// `requires-bound`.
    RequiresBound,
}

/// Negotiate each item independently against `backend`.
pub fn negotiate_ieee(
    items: &[IeeeItemRequirement],
    backend: &IeeeBackendCapabilities,
) -> Vec<IeeeDisposition> {
    items
        .iter()
        .map(|item| {
            let unsupported = if !backend.widths.contains(&item.width) {
                Some(IeeeUnsupportedCause::Width(item.width))
            } else if !backend.operations.contains(&item.operation) {
                Some(IeeeUnsupportedCause::Operation(item.operation))
            } else if item.operation.rounds() && !backend.roundings.contains(&item.rounding) {
                Some(IeeeUnsupportedCause::Rounding(item.rounding))
            } else if !backend.exceptional_policy {
                Some(IeeeUnsupportedCause::ExceptionalPolicy)
            } else {
                None
            };
            match unsupported {
                Some(cause) => IeeeDisposition::Unsupported(cause),
                None if item.requires_finite_proof && !backend.finite_proof => {
                    IeeeDisposition::RequiresBound
                }
                None => IeeeDisposition::Supported,
            }
        })
        .collect()
}

// ---- format -----------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Format {
    exponent_bits: u32,
    fraction_bits: u32,
}

impl Format {
    fn fraction(self) -> i64 {
        i64::from(self.fraction_bits)
    }

    fn bias(self) -> i64 {
        (1_i64 << (self.exponent_bits - 1)) - 1
    }

    fn emin(self) -> i64 {
        1 - self.bias()
    }

    fn emax(self) -> i64 {
        self.bias()
    }

    /// Exponent of the least subnormal quantum.
    fn quantum_min(self) -> i64 {
        self.emin() - self.fraction()
    }

    fn sign_mask(self) -> u64 {
        1_u64 << (self.exponent_bits + self.fraction_bits)
    }

    fn all_mask(self) -> u64 {
        (self.sign_mask() - 1) | self.sign_mask()
    }

    fn exponent_ones(self) -> u64 {
        (1_u64 << self.exponent_bits) - 1
    }

    fn exponent_mask(self) -> u64 {
        self.exponent_ones() << self.fraction_bits
    }

    fn fraction_mask(self) -> u64 {
        (1_u64 << self.fraction_bits) - 1
    }

    fn quiet_bit(self) -> u64 {
        1_u64 << (self.fraction_bits - 1)
    }

    fn sign(self, negative: bool) -> u64 {
        if negative {
            self.sign_mask()
        } else {
            0
        }
    }

    fn infinity(self, negative: bool) -> u64 {
        self.sign(negative) | self.exponent_mask()
    }

    fn max_finite(self, negative: bool) -> u64 {
        self.sign(negative)
            | (self.exponent_mask() - (1_u64 << self.fraction_bits))
            | self.fraction_mask()
    }

    fn zero(self, negative: bool) -> u64 {
        self.sign(negative)
    }

    fn canonical_nan(self) -> u64 {
        self.exponent_mask() | self.quiet_bit()
    }
}

/// A finite nonzero operand `(-1)^negative × significand × 2^exponent`.
#[derive(Clone, Copy, Debug)]
struct Finite {
    negative: bool,
    significand: u64,
    exponent: i64,
}

impl Finite {
    /// `maxparts` of the reduced exact value, computed from the decoded fields
    /// without materializing it: `odd × 2^e` with `odd` odd has parts
    /// `(odd << e, 1)` for `e ≥ 0` and `(odd, 2^-e)` otherwise.
    fn max_part_bits(self) -> u64 {
        let zeros = self.significand.trailing_zeros();
        let odd = self.significand >> zeros;
        let odd_bits = u64::from(u64::BITS - odd.leading_zeros());
        let exponent = self.exponent + i64::from(zeros);
        if exponent >= 0 {
            odd_bits + exponent.unsigned_abs()
        } else {
            odd_bits.max(exponent.unsigned_abs() + 1)
        }
    }

    fn to_rational(self) -> Rational {
        let magnitude = BigInt::from(self.significand);
        let signed = if self.negative { -magnitude } else { magnitude };
        let shift = self.exponent.unsigned_abs();
        if self.exponent >= 0 {
            Rational::from_integer(Integer::from_big(signed << shift))
        } else {
            Rational::from_integer(Integer::from_big(signed)).divided_by_power_of_two(shift)
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Class {
    Nan { negative: bool, signaling: bool },
    Infinite { negative: bool },
    Zero { negative: bool },
    Finite(Finite),
}

fn decode(value: IeeeValue) -> Class {
    let format = value.width.format();
    let negative = value.bits & format.sign_mask() != 0;
    // The mask leaves at most eleven bits, so the low two bytes hold the whole
    // field and the projection to `u16` is exact.
    let [.., high, low] =
        ((value.bits >> format.fraction_bits) & format.exponent_ones()).to_be_bytes();
    let biased = u16::from_be_bytes([high, low]);
    let fraction = value.bits & format.fraction_mask();
    if u64::from(biased) == format.exponent_ones() {
        if fraction == 0 {
            Class::Infinite { negative }
        } else {
            Class::Nan {
                negative,
                signaling: fraction & format.quiet_bit() == 0,
            }
        }
    } else if biased == 0 {
        if fraction == 0 {
            Class::Zero { negative }
        } else {
            Class::Finite(Finite {
                negative,
                significand: fraction,
                exponent: format.quantum_min(),
            })
        }
    } else {
        Class::Finite(Finite {
            negative,
            significand: fraction | (1_u64 << format.fraction_bits),
            exponent: i64::from(biased) - format.bias() - format.fraction(),
        })
    }
}

fn same_width(first: IeeeValue, rest: &[IeeeValue]) -> Result<IeeeWidth, IllTyped> {
    if rest.iter().all(|operand| operand.width == first.width) {
        Ok(first.width)
    } else {
        Err(IllTyped {
            cause: IllTypedCause::DistinctIeeeWidths,
        })
    }
}

// ---- accounting -------------------------------------------------------------

fn width_bits(width: IeeeWidth) -> u64 {
    u64::from(width.bits())
}

fn charge_operands(meter: &mut Meter, width: IeeeWidth, arity: u64) -> Result<(), Stop> {
    charge_operand_bits(meter, width_bits(width), arity)
}

fn charge_operand_bits(meter: &mut Meter, integer_bits: u64, arity: u64) -> Result<(), Stop> {
    meter.charge(
        Charge::new(ChargePoint::IeeeOperands)
            .size(LimitKind::IntegerBits, integer_bits)
            .size(LimitKind::ValueOccurrences, arity),
    )?;
    Ok(())
}

fn charge_width(meter: &mut Meter, point: ChargePoint, width: IeeeWidth) -> Result<(), Stop> {
    meter.charge(Charge::new(point).size(LimitKind::IntegerBits, width_bits(width)))?;
    Ok(())
}

fn charge_exact_result(meter: &mut Meter, max_part_bits: u64) -> Result<(), Stop> {
    meter.charge(
        Charge::new(ChargePoint::IeeeExactIntermediate).size(LimitKind::IntegerBits, max_part_bits),
    )?;
    Ok(())
}

fn charge_result(meter: &mut Meter) -> Result<(), Stop> {
    meter.charge(Charge::new(ChargePoint::IeeeResultRetain).results(1))?;
    Ok(())
}

// ---- evaluation ---------------------------------------------------------------

/// A value determined by classification alone: no exact real, no rounding.
struct Classified {
    bits: u64,
    flags: IeeeFlags,
}

impl Classified {
    fn exact(bits: u64) -> Self {
        Self {
            bits,
            flags: IeeeFlags::EMPTY,
        }
    }

    fn invalid(format: Format) -> Self {
        Self {
            bits: format.canonical_nan(),
            flags: IeeeFlags::EMPTY.with(IeeeFlag::Invalid),
        }
    }
}

/// An operand after classification: an infinity or a finite (possibly zero)
/// dyadic value.
#[derive(Clone, Copy, Debug)]
enum Operand {
    Infinite { negative: bool },
    Zero { negative: bool },
    Finite(Finite),
}

impl Operand {
    fn negative(self) -> bool {
        match self {
            Self::Infinite { negative } | Self::Zero { negative } => negative,
            Self::Finite(finite) => finite.negative,
        }
    }

    fn negated(self) -> Self {
        match self {
            Self::Infinite { negative } => Self::Infinite {
                negative: !negative,
            },
            Self::Zero { negative } => Self::Zero {
                negative: !negative,
            },
            Self::Finite(finite) => Self::Finite(Finite {
                negative: !finite.negative,
                ..finite
            }),
        }
    }
}

fn arithmetic(
    operation: IeeeOperation,
    width: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Result<IeeeResult, Stop> {
    let format = width.format();
    charge_operands(meter, width, operation.arity())?;
    let provenance = IeeeProvenance { width, rounding };
    let plan = match operation {
        IeeeOperation::Add(a, b) => classify(format, [a, b]).map(|[a, b]| plan_add(format, a, b)),
        IeeeOperation::Subtract(a, b) => {
            classify(format, [a, b]).map(|[a, b]| plan_add(format, a, b.negated()))
        }
        IeeeOperation::Multiply(a, b) => {
            classify(format, [a, b]).map(|[a, b]| plan_multiply(format, a, b))
        }
        IeeeOperation::Divide(a, b) => {
            classify(format, [a, b]).map(|[a, b]| plan_divide(format, a, b))
        }
        IeeeOperation::SquareRoot(a) => {
            classify(format, [a]).map(|[a]| plan_square_root(format, a))
        }
        IeeeOperation::FusedMultiplyAdd(a, b, c) => {
            classify(format, [a, b, c]).map(|[a, b, c]| plan_fma(format, a, b, c))
        }
    }
    .unwrap_or_else(Plan::Classified);
    match plan {
        Plan::Classified(classified) => retain_classified(meter, classified, width, provenance),
        Plan::Finite(finite) => {
            charge_width(meter, ChargePoint::IeeeExactIntermediate, width)?;
            let exact = finite.exact(format, rounding);
            finish_rounding(meter, exact, width, rounding)
        }
    }
}

/// Decode operands in order. Any NaN determines the result: the leftmost NaN,
/// quieted with its sign and payload kept, raising `invalid` when any operand
/// is signaling.
fn classify<const N: usize>(
    format: Format,
    values: [IeeeValue; N],
) -> Result<[Operand; N], Classified> {
    let mut operands = [Operand::Zero { negative: false }; N];
    let mut nan: Option<(u64, bool)> = None;
    for (slot, value) in operands.iter_mut().zip(values) {
        match decode(value) {
            Class::Nan { signaling, .. } => {
                let leftmost = nan.get_or_insert((value.bits, false));
                leftmost.1 |= signaling;
            }
            Class::Infinite { negative } => *slot = Operand::Infinite { negative },
            Class::Zero { negative } => *slot = Operand::Zero { negative },
            Class::Finite(finite) => *slot = Operand::Finite(finite),
        }
    }
    match nan {
        Some((bits, signaling)) => Err(Classified {
            bits: bits | format.quiet_bit(),
            flags: if signaling {
                IeeeFlags::EMPTY.with(IeeeFlag::Invalid)
            } else {
                IeeeFlags::EMPTY
            },
        }),
        None => Ok(operands),
    }
}

fn retain_classified(
    meter: &mut Meter,
    classified: Classified,
    width: IeeeWidth,
    provenance: IeeeProvenance,
) -> Result<IeeeResult, Stop> {
    charge_result(meter)?;
    Ok(IeeeResult {
        value: IeeeValue {
            width,
            bits: classified.bits,
        },
        flags: classified.flags,
        provenance,
    })
}

/// Charge `ieee.round`, round once, apply strict `exact`, then retain.
fn finish_rounding(
    meter: &mut Meter,
    exact: Exact,
    width: IeeeWidth,
    rounding: RoundingMode,
) -> Result<IeeeResult, Stop> {
    charge_width(meter, ChargePoint::IeeeRound, width)?;
    let (bits, flags) = round(width.format(), exact, rounding);
    // `value-accounting.md`: strict `exact` refuses after `ieee.round` and
    // before `ieee.result-retain`.
    if rounding == RoundingMode::Exact && !flags.is_empty() {
        return Err(Stop::Refused(Refusal::IeeeNotExact { would_be: flags }));
    }
    charge_result(meter)?;
    Ok(IeeeResult {
        value: IeeeValue { width, bits },
        flags,
        provenance: IeeeProvenance { width, rounding },
    })
}

enum Plan {
    Classified(Classified),
    Finite(FiniteOperation),
}

// `value-accounting.md`: an operation whose operands are all finite, zeros of
// either sign included, and that is neither invalid nor divide-by-zero charges
// `ieee.exact-intermediate` and `ieee.round`.
enum FiniteOperation {
    Add(Operand, Operand),
    Multiply(Operand, Operand),
    Divide(Operand, Finite),
    SquareRoot(Operand),
    FusedMultiplyAdd(Operand, Operand, Operand),
}

fn plan_add(format: Format, a: Operand, b: Operand) -> Plan {
    match (a, b) {
        (Operand::Infinite { negative: x }, Operand::Infinite { negative: y }) => {
            Plan::Classified(if x == y {
                Classified::exact(format.infinity(x))
            } else {
                Classified::invalid(format)
            })
        }
        (Operand::Infinite { negative }, _) | (_, Operand::Infinite { negative }) => {
            Plan::Classified(Classified::exact(format.infinity(negative)))
        }
        _ => Plan::Finite(FiniteOperation::Add(a, b)),
    }
}

fn plan_multiply(format: Format, a: Operand, b: Operand) -> Plan {
    let negative = a.negative() != b.negative();
    match (a, b) {
        (Operand::Infinite { .. }, Operand::Zero { .. })
        | (Operand::Zero { .. }, Operand::Infinite { .. }) => {
            Plan::Classified(Classified::invalid(format))
        }
        (Operand::Infinite { .. }, _) | (_, Operand::Infinite { .. }) => {
            Plan::Classified(Classified::exact(format.infinity(negative)))
        }
        _ => Plan::Finite(FiniteOperation::Multiply(a, b)),
    }
}

fn plan_divide(format: Format, a: Operand, b: Operand) -> Plan {
    let negative = a.negative() != b.negative();
    match (a, b) {
        (Operand::Infinite { .. }, Operand::Infinite { .. })
        | (Operand::Zero { .. }, Operand::Zero { .. }) => {
            Plan::Classified(Classified::invalid(format))
        }
        (Operand::Infinite { .. }, _) => {
            Plan::Classified(Classified::exact(format.infinity(negative)))
        }
        (_, Operand::Infinite { .. }) => Plan::Classified(Classified::exact(format.zero(negative))),
        (Operand::Finite(_), Operand::Zero { .. }) => Plan::Classified(Classified {
            bits: format.infinity(negative),
            flags: IeeeFlags::EMPTY.with(IeeeFlag::DivideByZero),
        }),
        (_, Operand::Finite(divisor)) => Plan::Finite(FiniteOperation::Divide(a, divisor)),
    }
}

fn plan_square_root(format: Format, a: Operand) -> Plan {
    match a {
        Operand::Infinite { negative: false } => {
            Plan::Classified(Classified::exact(format.infinity(false)))
        }
        Operand::Infinite { negative: true } | Operand::Finite(Finite { negative: true, .. }) => {
            Plan::Classified(Classified::invalid(format))
        }
        Operand::Zero { .. } | Operand::Finite(_) => Plan::Finite(FiniteOperation::SquareRoot(a)),
    }
}

fn plan_fma(format: Format, a: Operand, b: Operand, c: Operand) -> Plan {
    let product_negative = a.negative() != b.negative();
    let product_infinite = match (a, b) {
        (Operand::Infinite { .. }, Operand::Zero { .. })
        | (Operand::Zero { .. }, Operand::Infinite { .. }) => {
            return Plan::Classified(Classified::invalid(format))
        }
        (Operand::Infinite { .. }, _) | (_, Operand::Infinite { .. }) => true,
        _ => false,
    };
    match (product_infinite, c) {
        (true, Operand::Infinite { negative }) if negative != product_negative => {
            Plan::Classified(Classified::invalid(format))
        }
        (true, _) => Plan::Classified(Classified::exact(format.infinity(product_negative))),
        (false, Operand::Infinite { negative }) => {
            Plan::Classified(Classified::exact(format.infinity(negative)))
        }
        (false, _) => Plan::Finite(FiniteOperation::FusedMultiplyAdd(a, b, c)),
    }
}

/// The exact real intermediate of a finite operation.
enum Exact {
    /// An exact zero with its IEEE sign.
    Zero { negative: bool },
    /// A nonzero real known through a rounding-sufficient approximation.
    Real {
        negative: bool,
        approximation: Approximation,
    },
}

/// A positive real `x` held two guard bits below its rounding quantum:
/// `x = (q + f) × 2^(quantum - 2)` with `f = 0` exactly when `sticky` is false
/// and `0 < f < 1` otherwise.
///
/// `quantum = max(log2 - p, quantum_min)` is the exponent of the least
/// significant retained bit; only its non-negative offset
/// `index = quantum - quantum_min` is kept, with `log2 = floor(log2(x))` for
/// tininess. Every
/// value is built by [`Approximation::on_grid`], which establishes
/// `q < 2^(p+3)`.
struct Approximation {
    q: u64,
    sticky: bool,
    index: u64,
    log2: i64,
}

/// How many guard bits beneath a rounding position are discarded.
#[derive(Clone, Copy)]
enum Guard {
    /// Round at `quantum - 1` (the unbounded-exponent position of a real just
    /// below `2^emin`).
    One,
    /// Round at `quantum`.
    Two,
}

impl Guard {
    fn shift(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }

    fn half(self) -> u64 {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }
}

impl Approximation {
    /// Place the real with `floor(log2(x)) = log2` on its rounding grid.
    ///
    /// `scaled(target)` must return `floor(x / 2^target)` and whether that
    /// quotient is exact.
    fn on_grid(format: Format, log2: i64, scaled: impl FnOnce(i64) -> (BigUint, bool)) -> Self {
        let above = log2 - format.fraction() - format.quantum_min();
        let (quantum, index) = if above > 0 {
            (log2 - format.fraction(), above.unsigned_abs())
        } else {
            (format.quantum_min(), 0)
        };
        let (q, exact) = scaled(quantum - 2);
        // Proof that `q` fits: `x < 2^(log2 + 1)`, and both branches above
        // give `quantum ≥ log2 - p` (the first with equality, the second
        // because `above ≤ 0`). So `q ≤ x / 2^(quantum - 2) < 2^(p + 3)`,
        // and `p ≤ 52` bounds it by `2^55`.
        let q = u64::try_from(q).expect("q < 2^(p+3) by the grid choice above");
        Self {
            q,
            sticky: !exact,
            index,
            log2,
        }
    }
}

fn signed_dyadic(operand: Operand) -> (BigInt, i64) {
    match operand {
        Operand::Finite(finite) => {
            let magnitude = BigInt::from(finite.significand);
            (
                if finite.negative {
                    -magnitude
                } else {
                    magnitude
                },
                finite.exponent,
            )
        }
        Operand::Zero { .. } | Operand::Infinite { .. } => (BigInt::zero(), 0),
    }
}

/// Bit length of a `u64`, in the signed exponent domain.
fn bit_length(value: u64) -> i64 {
    i64::from(u64::BITS - value.leading_zeros())
}

impl FiniteOperation {
    fn exact(self, format: Format, rounding: RoundingMode) -> Exact {
        match self {
            Self::Add(a, b) => sum(format, rounding, dyadic_term(a), dyadic_term(b)),
            Self::Multiply(a, b) => {
                let negative = a.negative() != b.negative();
                let ((x, ex), (y, ey)) = (signed_dyadic(a), signed_dyadic(b));
                real_from(format, negative, x * y, ex + ey)
            }
            Self::Divide(a, divisor) => {
                let negative = a.negative() != divisor.negative;
                match a {
                    Operand::Finite(dividend) => Exact::Real {
                        negative,
                        approximation: approximate(
                            &BigUint::from(dividend.significand),
                            &BigUint::from(divisor.significand),
                            dividend.exponent - divisor.exponent,
                            format,
                        ),
                    },
                    Operand::Zero { .. } | Operand::Infinite { .. } => Exact::Zero { negative },
                }
            }
            Self::SquareRoot(a) => match a {
                Operand::Finite(finite) => Exact::Real {
                    negative: false,
                    approximation: square_root(format, finite),
                },
                Operand::Zero { negative } | Operand::Infinite { negative } => {
                    Exact::Zero { negative }
                }
            },
            Self::FusedMultiplyAdd(a, b, c) => {
                let ((x, ex), (y, ey)) = (signed_dyadic(a), signed_dyadic(b));
                let product = Term {
                    negative: a.negative() != b.negative(),
                    value: x * y,
                    exponent: ex + ey,
                };
                sum(format, rounding, product, dyadic_term(c))
            }
        }
    }
}

/// A signed exact dyadic `value × 2^exponent` whose zero carries an IEEE sign.
struct Term {
    negative: bool,
    value: BigInt,
    exponent: i64,
}

fn dyadic_term(operand: Operand) -> Term {
    let (value, exponent) = signed_dyadic(operand);
    Term {
        negative: operand.negative(),
        value,
        exponent,
    }
}

fn sum(format: Format, rounding: RoundingMode, left: Term, right: Term) -> Exact {
    if left.value.is_zero() && right.value.is_zero() {
        let negative = if left.negative == right.negative {
            left.negative
        } else {
            rounding == RoundingMode::TowardNegative
        };
        return Exact::Zero { negative };
    }
    let exponent = left.exponent.min(right.exponent);
    let align = |term: Term| term.value << (term.exponent - exponent).unsigned_abs();
    let total = align(left) + align(right);
    if total.is_zero() {
        return Exact::Zero {
            negative: rounding == RoundingMode::TowardNegative,
        };
    }
    let negative = total.sign() == Sign::Minus;
    real_from(format, negative, total, exponent)
}

fn real_from(format: Format, negative: bool, value: BigInt, exponent: i64) -> Exact {
    if value.is_zero() {
        return Exact::Zero { negative };
    }
    Exact::Real {
        negative,
        approximation: approximate(value.magnitude(), &BigUint::one(), exponent, format),
    }
}

/// Approximate the positive real `numerator / denominator × 2^exponent`.
///
/// Bit lengths of arbitrary exact inputs are compared in `i128`, where they
/// cannot overflow. A real whose logarithm lies outside the format's rounding
/// range is replaced by a representative with the identical rounded result:
/// beyond `2^(emax+2)` every direction overflows exactly as for `2^(emax+2)`
/// plus a sticky fraction, and below `2^(quantum_min-2)` every direction
/// rounds exactly as for a sticky fraction of that quantum.
fn approximate(
    numerator: &BigUint,
    denominator: &BigUint,
    exponent: i64,
    format: Format,
) -> Approximation {
    let difference = i128::from(numerator.bits()) - i128::from(denominator.bits());
    let shift = difference.unsigned_abs();
    let at_least = if difference >= 0 {
        *numerator >= denominator << shift
    } else {
        numerator << shift >= *denominator
    };
    let log2 = if at_least { difference } else { difference - 1 } + i128::from(exponent);
    let (least, greatest) = (format.quantum_min() - 3, format.emax() + 2);
    match i64::try_from(log2) {
        Ok(log2) if log2 > least && log2 < greatest => {
            Approximation::on_grid(format, log2, |target| {
                let scale = exponent - target;
                let (scaled_numerator, scaled_denominator) = if scale >= 0 {
                    (numerator << scale.unsigned_abs(), denominator.clone())
                } else {
                    (numerator.clone(), denominator << scale.unsigned_abs())
                };
                let (q, remainder) = scaled_numerator.div_rem(&scaled_denominator);
                (q, remainder.is_zero())
            })
        }
        Ok(log2) if log2 <= least => tiny(format),
        Ok(_) => huge(format),
        Err(_) if log2 < 0 => tiny(format),
        Err(_) => huge(format),
    }
}

/// A sticky fraction of `2^(quantum_min - 2)`: rounds like any smaller real.
fn tiny(format: Format) -> Approximation {
    Approximation::on_grid(format, format.quantum_min() - 3, |_| {
        (BigUint::zero(), false)
    })
}

/// `2^(emax+2)` plus a sticky fraction: overflows like any larger real.
fn huge(format: Format) -> Approximation {
    Approximation::on_grid(format, format.emax() + 2, |_| {
        (BigUint::one() << (format.fraction_bits + 2), false)
    })
}

fn integer_square_root(value: &BigUint) -> BigUint {
    if value.is_zero() {
        return BigUint::zero();
    }
    // `value < 2^bits`, so `2^(bits/2 + 1)` is above its square root and
    // Newton's iteration descends to the floor.
    let mut estimate = BigUint::one() << (value.bits() / 2 + 1);
    loop {
        let next = (&estimate + value / &estimate) >> 1_u32;
        if next >= estimate {
            return estimate;
        }
        estimate = next;
    }
}

/// Approximate `sqrt(significand × 2^exponent)` of a positive finite operand.
///
/// For `y` in `[2^L, 2^(L+1))` the root's logarithm floor is `floor(L/2)`, and
/// `floor(sqrt(y)) = isqrt(floor(y))`, so the quotient needs only integers.
fn square_root(format: Format, operand: Finite) -> Approximation {
    let radicand = BigUint::from(operand.significand);
    let log2 = (bit_length(operand.significand) - 1 + operand.exponent).div_euclid(2);
    Approximation::on_grid(format, log2, |target| {
        // `root / 2^target = sqrt(radicand × 2^(exponent - 2·target))`.
        let scale = operand.exponent - 2 * target;
        let shift = scale.unsigned_abs();
        let (whole, dropped) = if scale >= 0 {
            (&radicand << shift, false)
        } else {
            let whole = &radicand >> shift;
            let dropped = &whole << shift != radicand;
            (whole, dropped)
        };
        let q = integer_square_root(&whole);
        let exact = !dropped && &q * &q == whole;
        (q, exact)
    })
}

/// Round `approximation` onto the grid `2^quantum` under a rounding
/// direction; returns the integer multiple and whether anything was discarded.
fn round_at(
    approximation: &Approximation,
    guard: Guard,
    negative: bool,
    direction: RoundingMode,
) -> (u64, bool) {
    let kept = approximation.q >> guard.shift();
    let remainder = approximation.q - (kept << guard.shift());
    let position = match remainder.cmp(&guard.half()) {
        Ordering::Equal if approximation.sticky => Ordering::Greater,
        ordering => ordering,
    };
    let inexact = remainder != 0 || approximation.sticky;
    let up = match direction {
        RoundingMode::Exact | RoundingMode::NearestEven => {
            position == Ordering::Greater || (position == Ordering::Equal && kept % 2 == 1)
        }
        RoundingMode::NearestAway => position != Ordering::Less,
        RoundingMode::TowardZero => false,
        RoundingMode::TowardPositive => inexact && !negative,
        RoundingMode::TowardNegative => inexact && negative,
    };
    // `kept < 2^(p+1)` because `q < 2^(p+3)`, so the increment cannot wrap.
    (if up { kept + 1 } else { kept }, inexact)
}

/// Round once and encode, with the fresh flag set.
///
/// Strict `exact` computes its would-be flags under nearest-even.
fn round(format: Format, exact: Exact, rounding: RoundingMode) -> (u64, IeeeFlags) {
    let (negative, approximation) = match exact {
        Exact::Zero { negative } => return (format.zero(negative), IeeeFlags::EMPTY),
        Exact::Real {
            negative,
            approximation,
        } => (negative, approximation),
    };
    // FR-148-AC-8: strict `exact` reports `nearest-even` would-be flags.
    let direction = match rounding {
        RoundingMode::Exact => RoundingMode::NearestEven,
        other => other,
    };
    let precision = format.fraction();
    let log2 = approximation.log2;
    let mut index = approximation.index;
    let (mut kept, inexact) = round_at(&approximation, Guard::Two, negative, direction);
    if bit_length(kept) > precision + 1 {
        kept >>= 1_u32;
        index += 1;
    }
    let mut flags = IeeeFlags::EMPTY;
    if inexact {
        flags = flags.with(IeeeFlag::Inexact);
    }
    // A normal result's biased exponent is `index + 1`; the all-ones field is
    // reserved, so reaching it overflows.
    if bit_length(kept) == precision + 1 && index + 1 >= format.exponent_ones() {
        let to_infinity = match direction {
            RoundingMode::Exact | RoundingMode::NearestEven | RoundingMode::NearestAway => true,
            RoundingMode::TowardZero => false,
            RoundingMode::TowardPositive => !negative,
            RoundingMode::TowardNegative => negative,
        };
        let bits = if to_infinity {
            format.infinity(negative)
        } else {
            format.max_finite(negative)
        };
        return (bits, flags.with(IeeeFlag::Overflow).with(IeeeFlag::Inexact));
    }
    // Tininess after rounding: the result rounded with an unbounded exponent
    // range lies strictly below the least normal magnitude `2^emin`.
    // When `log2 = emin - 1` the grid quantum is `quantum_min`, one above that
    // unbounded position, so one guard bit is discarded.
    let tiny = log2 < format.emin()
        && !(log2 + 1 == format.emin()
            && bit_length(round_at(&approximation, Guard::One, negative, direction).0)
                > precision + 1);
    if tiny && inexact {
        flags = flags.with(IeeeFlag::Underflow);
    }
    // A normal `kept` carries the implicit leading bit `2^p`, which adds the
    // final `1` to the biased exponent `index + 1`. A subnormal `kept` below
    // `2^p` only occurs at `index = 0` and encodes with a zero exponent field.
    let bits = format.sign(negative) | ((index << format.fraction_bits) + kept);
    (bits, flags)
}

fn convert_width(
    value: IeeeValue,
    target: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Result<IeeeResult, Stop> {
    charge_operands(meter, value.width, 1)?;
    let (source, format) = (value.width.format(), target.format());
    let provenance = IeeeProvenance {
        width: target,
        rounding,
    };
    let classified = match decode(value) {
        Class::Nan {
            negative,
            signaling,
        } => {
            // FR-148: the payload is the integer below the quiet bit, kept
            // unchanged; one not smaller than the target's quiet bit is refused
            // with no flags, before the NaN is consumed.
            let payload = value.bits & (source.quiet_bit() - 1);
            if payload >= format.quiet_bit() {
                return Err(Stop::Refused(Refusal::IeeeNanPayloadNotRepresentable));
            }
            Classified {
                bits: format.sign(negative) | format.canonical_nan() | payload,
                flags: if signaling {
                    IeeeFlags::EMPTY.with(IeeeFlag::Invalid)
                } else {
                    IeeeFlags::EMPTY
                },
            }
        }
        Class::Infinite { negative } => Classified::exact(format.infinity(negative)),
        Class::Zero { negative } => {
            charge_width(meter, ChargePoint::IeeeExactIntermediate, target)?;
            return finish_rounding(meter, Exact::Zero { negative }, target, rounding);
        }
        Class::Finite(finite) => {
            charge_width(meter, ChargePoint::IeeeExactIntermediate, target)?;
            let exact = Exact::Real {
                negative: finite.negative,
                approximation: approximate(
                    &BigUint::from(finite.significand),
                    &BigUint::one(),
                    finite.exponent,
                    format,
                ),
            };
            return finish_rounding(meter, exact, target, rounding);
        }
    };
    retain_classified(meter, classified, target, provenance)
}
