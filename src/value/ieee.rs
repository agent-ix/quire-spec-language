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
use super::decimal::RoundingMode;
use super::definition::{
    CatalogRole, DefinitionLock, DefinitionReference, PackageCause, PackageRefusal,
    PackageRefusalCode,
};
use super::integer::Integer;
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::Rational;

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

/// An arithmetic operation over same-width operands.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IeeeOperation {
    /// `a + b`.
    Add(IeeeValue, IeeeValue),
    /// `a - b`.
    Subtract(IeeeValue, IeeeValue),
    /// `a * b`.
    Multiply(IeeeValue, IeeeValue),
    /// `a / b`.
    Divide(IeeeValue, IeeeValue),
    /// `quire::value::ieee::sqrt(a)`.
    SquareRoot(IeeeValue),
    /// `quire::value::ieee::fma(a, b, c)`: `a * b + c` with one rounding.
    FusedMultiplyAdd(IeeeValue, IeeeValue, IeeeValue),
}

impl IeeeOperation {
    /// The operation's kind.
    pub fn kind(self) -> IeeeOperationKind {
        match self {
            Self::Add(..) => IeeeOperationKind::Add,
            Self::Subtract(..) => IeeeOperationKind::Subtract,
            Self::Multiply(..) => IeeeOperationKind::Multiply,
            Self::Divide(..) => IeeeOperationKind::Divide,
            Self::SquareRoot(..) => IeeeOperationKind::SquareRoot,
            Self::FusedMultiplyAdd(..) => IeeeOperationKind::FusedMultiplyAdd,
        }
    }

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

/// The exact value of a finite IEEE operand.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IeeeExact {
    value: Rational,
    discarded_negative_zero: bool,
}

impl IeeeExact {
    /// The exact rational.
    pub fn value(&self) -> &Rational {
        &self.value
    }

    /// Whether the source was `-0`, whose sign has no rational representation
    /// and is reported as loss.
    pub fn discarded_negative_zero(&self) -> bool {
        self.discarded_negative_zero
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
        let expected = &self
            .entry(CatalogRole::IeeeProfile)
            .ok_or(invalid_package(PackageCause::MissingMember))?
            .definition;
        for reference in retained {
            let cause = if reference.identity != expected.identity
                || reference.authority != expected.authority
            {
                Some(PackageCause::IncompatibleDefinition)
            } else if reference.revision != expected.revision {
                Some(PackageCause::RevisionMismatch)
            } else if reference.digest_domain != expected.digest_domain {
                Some(PackageCause::DigestDomainMismatch)
            } else if reference.digest != expected.digest {
                Some(PackageCause::ByteDigestMismatch)
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
        // SPEC-GAP(ieee-6): TC-193 names only `invalid_package` for a user
        // declaration bound to a reserved intrinsic identity; the closed I04
        // cause vocabulary has no reserved-identity tag, so the closest
        // `conflicting-definition` is reported.
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
/// Operands of different widths are ill-typed and consume nothing. Otherwise the
/// result is completed bits and flags, a strict-`exact` refusal carrying only
/// the would-be flags, or incomplete with no bits or flags.
pub fn evaluate_ieee(
    _profile: &AdmittedIeeeProfile,
    operation: IeeeOperation,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Result<Outcome<IeeeResult>, IllTyped> {
    let (first, rest) = operation.operands();
    let width = same_width(first, &rest)?;
    Ok(Outcome::from_stop(arithmetic(
        operation, width, rounding, meter,
    )))
}

/// Evaluate one comparison intrinsic. Cross-width operands are ill-typed and
/// consume nothing.
pub fn compare_ieee(
    _profile: &AdmittedIeeeProfile,
    comparison: IeeeComparison,
    left: IeeeValue,
    right: IeeeValue,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    // SPEC-GAP(ieee-1): FR-148 says `bitIdentical` "requires the same width"
    // and TC-193 F05 says bit identity "follows width plus bits", while F06
    // makes cross-width comparison ill-typed. All three intrinsics are read as
    // ill-typed across widths, so no Boolean ever depends on a width mismatch.
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

/// Explicitly convert a finite IEEE value to its exact rational. NaN and the
/// infinities have no exact value and are undefined.
pub fn ieee_to_exact(
    _profile: &AdmittedIeeeProfile,
    value: IeeeValue,
    meter: &mut Meter,
) -> Outcome<IeeeExact> {
    Outcome::from_stop(to_exact(value, meter))
}

fn to_exact(value: IeeeValue, meter: &mut Meter) -> Result<IeeeExact, Stop> {
    // SPEC-GAP(ieee-5): `value-accounting.md` names no conversion charges.
    // Conversions use the IEEE family as unary operations: `ieee.operands`
    // at the source width, and for a rounding conversion
    // `ieee.exact-intermediate`, `ieee.round` and `ieee.result-retain` at
    // the target width.
    charge_operands(meter, value.width, 1)?;
    let exact = match decode(value) {
        Class::Nan { .. } | Class::Infinite { .. } => {
            return Err(Stop::Undefined(Undefined::IeeeNotFinite))
        }
        Class::Zero { negative } => IeeeExact {
            value: Rational::from_integer(Integer::zero()),
            discarded_negative_zero: negative,
        },
        Class::Finite(finite) => IeeeExact {
            value: finite.to_rational(),
            discarded_negative_zero: false,
        },
    };
    charge_result(meter)?;
    Ok(exact)
}

/// Explicitly convert an exact rational to `width` under `rounding`, reporting
/// loss through the IEEE flags; strict `exact` refuses any loss.
pub fn exact_to_ieee(
    _profile: &AdmittedIeeeProfile,
    value: &Rational,
    width: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Outcome<IeeeResult> {
    Outcome::from_stop(from_exact(value, width, rounding, meter))
}

fn from_exact(
    value: &Rational,
    width: IeeeWidth,
    rounding: RoundingMode,
    meter: &mut Meter,
) -> Result<IeeeResult, Stop> {
    charge_operands(meter, width, 1)?;
    charge_width(meter, ChargePoint::IeeeExactIntermediate, width)?;
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
    fn to_rational(self) -> Rational {
        let magnitude = BigInt::from(self.significand);
        let shift = self.exponent.unsigned_abs();
        let (numerator, denominator) = if self.exponent >= 0 {
            (magnitude << shift, BigInt::one())
        } else {
            (magnitude, BigInt::one() << shift)
        };
        let numerator = if self.negative { -numerator } else { numerator };
        Rational::new(Integer::from_big(numerator), Integer::from_big(denominator))
            .unwrap_or_else(|_| Rational::from_integer(Integer::zero()))
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
    let biased = (value.bits >> format.fraction_bits) & format.exponent_ones();
    let fraction = value.bits & format.fraction_mask();
    if biased == format.exponent_ones() {
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
            // `biased` is masked to at most eleven bits.
            exponent: i64::try_from(biased).unwrap_or(i64::MAX) - format.bias() - format.fraction(),
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
    meter.charge(
        Charge::new(ChargePoint::IeeeOperands)
            .size(LimitKind::IntegerBits, width_bits(width))
            .size(LimitKind::ValueOccurrences, arity),
    )?;
    Ok(())
}

fn charge_width(meter: &mut Meter, point: ChargePoint, width: IeeeWidth) -> Result<(), Stop> {
    meter.charge(Charge::new(point).size(LimitKind::IntegerBits, width_bits(width)))?;
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
    // SPEC-GAP(ieee-3): FR-148 does not place the strict-`exact` refusal among
    // the IEEE charges. Representability is decided by the rounding step, so
    // the refusal follows `ieee.round` and precedes `ieee.result-retain`.
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

// SPEC-GAP(ieee-2): `value-accounting.md` classifies results "determined by
// NaN, infinity, signed-zero or invalid/divide-by-zero classification" but also
// charges "finite add, subtract, multiply, divide and FMA". A zero operand is
// finite, so an operation whose operands are all finite (zeros included) and
// that is neither invalid nor divide-by-zero takes the finite path and charges
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

/// `(q + f) × 2^exponent` with `f = 0` exactly when `sticky` is false and
/// `0 < f < 1` otherwise, where `log2` is the floor of the real's base-two
/// logarithm. Construction guarantees `q ≥ 2^(p+1)` or
/// `exponent ≤ quantum_min - 2`, so every rounding position used below has at
/// least one bit of `q` beneath it.
struct Approximation {
    q: BigUint,
    exponent: i64,
    sticky: bool,
    log2: i64,
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

fn bit_length(value: &BigUint) -> i64 {
    i64::try_from(value.bits()).unwrap_or(i64::MAX)
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
fn approximate(
    numerator: &BigUint,
    denominator: &BigUint,
    exponent: i64,
    format: Format,
) -> Approximation {
    let difference = bit_length(numerator) - bit_length(denominator);
    let shift = difference.unsigned_abs();
    let at_least = if difference >= 0 {
        *numerator >= denominator << shift
    } else {
        numerator << shift >= *denominator
    };
    let log2 = if at_least { difference } else { difference - 1 } + exponent;
    let target = (log2 - format.fraction() - 2).max(format.quantum_min() - 2);
    let scale = exponent - target;
    let (scaled_numerator, scaled_denominator) = if scale >= 0 {
        (numerator << scale.unsigned_abs(), denominator.clone())
    } else {
        (numerator.clone(), denominator << scale.unsigned_abs())
    };
    let (q, remainder) = scaled_numerator.div_rem(&scaled_denominator);
    Approximation {
        q,
        exponent: target,
        sticky: !remainder.is_zero(),
        log2,
    }
}

fn integer_square_root(value: &BigUint) -> BigUint {
    if value.is_zero() {
        return BigUint::zero();
    }
    let mut estimate = BigUint::one() << (value.bits().saturating_add(1) / 2);
    loop {
        let next = (&estimate + value / &estimate) >> 1_u32;
        if next >= estimate {
            return estimate;
        }
        estimate = next;
    }
}

fn square_root(format: Format, operand: Finite) -> Approximation {
    let (mut radicand, mut exponent) = (BigUint::from(operand.significand), operand.exponent);
    if exponent.rem_euclid(2) != 0 {
        radicand <<= 1_u32;
        exponent -= 1;
    }
    let wanted = 2 * (format.fraction() + 3);
    let extra = ((wanted - bit_length(&radicand)).max(0) + 1) / 2;
    let scaled = radicand << (2 * extra).unsigned_abs();
    let q = integer_square_root(&scaled);
    let sticky = &q * &q != scaled;
    let exponent = exponent.div_euclid(2) - extra;
    Approximation {
        log2: bit_length(&q) - 1 + exponent,
        q,
        exponent,
        sticky,
    }
}

/// Round `approximation` onto the grid `2^quantum` under a rounding
/// direction; returns the integer multiple and whether anything was discarded.
fn round_at(
    approximation: &Approximation,
    quantum: i64,
    negative: bool,
    direction: RoundingMode,
) -> (BigUint, bool) {
    let shift = (quantum - approximation.exponent).max(1).unsigned_abs();
    let kept = &approximation.q >> shift;
    let remainder = &approximation.q - (&kept << shift);
    let half = BigUint::one() << (shift - 1);
    let position = match remainder.cmp(&half) {
        Ordering::Equal if approximation.sticky => Ordering::Greater,
        ordering => ordering,
    };
    let inexact = !remainder.is_zero() || approximation.sticky;
    let up = match direction {
        RoundingMode::Exact | RoundingMode::NearestEven => {
            position == Ordering::Greater || (position == Ordering::Equal && kept.is_odd())
        }
        RoundingMode::NearestAway => position != Ordering::Less,
        RoundingMode::TowardZero => false,
        RoundingMode::TowardPositive => inexact && !negative,
        RoundingMode::TowardNegative => inexact && negative,
    };
    (if up { kept + 1_u32 } else { kept }, inexact)
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
    // SPEC-GAP(ieee-4): FR-148 refuses a strict-`exact` result "with its
    // would-be flags" without naming the direction those flags assume; overflow
    // and tininess depend on it. Would-be flags use nearest-even.
    let direction = match rounding {
        RoundingMode::Exact => RoundingMode::NearestEven,
        other => other,
    };
    let precision = format.fraction();
    let log2 = approximation.log2;
    let mut quantum = (log2 - precision).max(format.quantum_min());
    let (mut kept, inexact) = round_at(&approximation, quantum, negative, direction);
    if bit_length(&kept) > precision + 1 {
        kept >>= 1_u32;
        quantum += 1;
    }
    let mut flags = IeeeFlags::EMPTY;
    if inexact {
        flags = flags.with(IeeeFlag::Inexact);
    }
    if bit_length(&kept) == precision + 1 && quantum + precision > format.emax() {
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
    let tiny = log2 < format.emin()
        && !(log2 + 1 == format.emin()
            && bit_length(&round_at(&approximation, log2 - precision, negative, direction).0)
                > precision + 1);
    if tiny && inexact {
        flags = flags.with(IeeeFlag::Underflow);
    }
    let magnitude = kept.iter_u64_digits().next().unwrap_or(0);
    let bits = if bit_length(&kept) <= precision {
        // Subnormal or zero: `quantum` is the least quantum.
        format.sign(negative) | magnitude
    } else {
        let biased = (quantum + precision + format.bias()).unsigned_abs();
        format.sign(negative)
            | (biased << format.fraction_bits)
            | (magnitude & format.fraction_mask())
    };
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
            // SPEC-GAP(ieee-7): FR-148 names explicit cross-width conversion
            // but no NaN payload mapping. The payload is the integer below the
            // quiet bit; it is kept unchanged, and a payload the target cannot
            // hold is refused instead of truncated.
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
